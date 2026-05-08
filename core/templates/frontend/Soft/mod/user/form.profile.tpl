<div class="clear-line"></div>
<style type="text/css">
    .form-area { border-top: 0px; }
</style>
<script>
$(document).ready(function() {
    $(".tab-content").hide();
    $("ul.tabs li:first").addClass("active").show();
    $(".tab-content:first").show();
    $("ul.tabs li").click(function() {
		$("ul.tabs li").removeClass("active");
		$(this).addClass("active");
		$(".tab-content").hide();
		var activeTab = $(this).find("a").attr("href");
		$(activeTab).fadeIn();
		return false;
    });
    $("#avatar_change").click(function() {
        $(".tab-content").hide();
        $("#tab_avatar").fadeIn();
        return false;
    });
    $("#tab_avatar img").click(function() {
        $("#avatar_inp").attr("value", $(this).attr("id"));
        $("#avatar_this").attr("src", $(this).attr("src"));
        $(".tab-content").hide();
        $(".tab-content:first").show();
        return false;
    });
});
</script>
<ul class="tabs">
    <li><a href="#tab1">{user_data}</a></li>
    <!--if:editpass:yes--><li><a href="#tab2">{chang_pass}</a></li><!--if-->
    <!--if:editmail:yes--><li><a href="#tab3">{chang_email}</a></li><!--if-->
</ul>
<div class="clear"></div>
<div class="form-area"> 
    <form action="{site_url}/index.php?dn=user" method="post">
    <div class="tab-content" id="tab1">
        <fieldset>
            <legend>{lang_avatar}</legend>
			{avatar}
        </fieldset>
        <fieldset>
            <legend>ICQ</legend>
            <input name="edit[icq]" maxlength="15" type="text" value="{icq}" />
            <img src="{site_url}/temp/{site_temp}/images/icon/info.gif" alt="{icq_hint}" alt="" />
        </fieldset>
        <fieldset>
            <legend>MSN</legend>
            <input name="edit[msn]" type="text" maxlength="50" value="{msn}" />
            <img src="{site_url}/temp/{site_temp}/images/icon/info.gif" alt="{msn_hint}" alt="" />
        </fieldset> 
        <fieldset>
            <legend>Skype</legend>
            <input name="edit[skype]" type="text" maxlength="50" value="{skype}" />
            <img src="{site_url}/temp/{site_temp}/images/icon/info.gif" alt="{skype_hint}" alt="" />
        </fieldset>
        <fieldset>
            <legend>{urlname}</legend>
            <input name="edit[www]" type="text" maxlength="50" value="{url}" />
            <img src="{site_url}/temp/{site_temp}/images/icon/info.gif" alt="{www_hint}" alt="" />
        </fieldset>
        {addit_fields}
        <!--buffer:field:0--><fieldset><legend{empty}>{req}{name}</legend>{field}</fieldset><!--buffer-->
        <!--buffer:apart:0--><div class="form-area-apart">{name}</div><!--buffer-->
        <div class="form-area-apart ac">
            <input name="to" value="redata" type="hidden" />
            <button type="submit" class="sub">{up_data}</button>
        </div>  
    </div>
    </form> 
    <!--if:editpass:yes-->     
    <form action="{site_url}/index.php?dn=user" method="post">
    <div class="tab-content" id="tab2">
        <fieldset>
            <legend>{pass}</legend>
            <input name="onepassw" size="30" type="password" maxlength="{maxpass}" />
            <img src="{site_url}/temp/{site_temp}/images/icon/info.gif" alt="{pass_hint}" alt="" />
        </fieldset>
        <fieldset>
            <legend>{re_pass}</legend>
            <input name="twopassw" size="30" type="password" maxlength="{maxpass}" />
        </fieldset>
        <div class="form-area-apart ac">
            <input name="to" value="repassw" type="hidden" />
            <button type="submit" class="sub">{chang_button_pass}</button>
        </div>
    </div>
    </form>
    <!--if-->
    <!--if:editmail:yes--> 
    <form action="{site_url}/index.php?dn=user" method="post">
    <div class="tab-content" id="tab3">
        <fieldset>
            <legend>{e_mail}</legend>
            <input name="edit[onemail]" size="30" type="text" maxlength="255" value="{umail}" />
            <img src="{site_url}/temp/{site_temp}/images/icon/info.gif" alt="{mail_hint}" alt="" />
        </fieldset>
        <fieldset>
            <legend>{re_e_mail}</legend>
            <input name="edit[twomail]" size="30" type="text" maxlength="255" value="{umail}" />
        </fieldset>
        <div class="form-area-apart ac">
            <input name="to" value="remail" type="hidden" />
            <button type="submit" class="sub">{chang_button_email}</button>
        </div>
    </div>
    </form>
    <!--if-->

    <div class="tab-content" id="tab_avatar">
        {avatarlist}
    </div>

</div>

<!--buffer:avatar_danneo:0-->
        <a class="avatar" id="avatar_change" href="#"><img id="avatar_this" src="{site_url}{src}" alt="{change}" /></a>
        <input type="hidden" name="edit[avatar]" value="{name}" id="avatar_inp" />
<!--buffer-->

<!--buffer:avatar_thumb:0-->
<div class="thumb-cet ac">
    <a href="#"><img id="{name}" src="{site_url}{avatar_src}" alt="{name}" /></a>
</div>
<!--buffer-->
